//! The plan tree: the plan itself, its front matter, staves, measures,
//! and voice lanes.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_score::{ChordSymbol, Clef, Meter, MusicalDuration, VoiceId};

use super::items::NotatedItem;
use super::marks::{
    ClefChange, HairpinRange, KeySignature, MarkRange, OpenMark, PhraseRange, PointMark, PositionedMark, RepeatMark,
    SlurRange, TempoText,
};

/// Options for notation planning. Empty until a real choice exists (beaming
/// preferences, part extraction); the type fixes the facade shape.
#[derive(Clone, Debug, Default)]
pub struct NotationOptions {}

/// The backend-neutral plan for one score.
#[derive(Clone, Debug)]
pub struct NotationPlan {
    pub(super) front: FrontMatter,
    pub(super) staves: Vec<StaffPlan>,
    pub(super) tempos: Vec<PositionedMark<TempoText>>,
    pub(super) sections: Vec<PositionedMark<String>>,
    pub(super) harmony: Vec<PositionedMark<ChordSymbol>>,
    pub(super) repeats: Vec<RepeatMark>,
    pub(super) open: Vec<OpenMark>,
    pub(super) holds: Vec<PositionedMark<musa_score::FreeDuration>>,
}

impl NotationPlan {
    /// What the page prints around the music: title, and the four optional
    /// lines an edition carries. Every backend writes these into its own
    /// header; none of them decides where on the paper they go.
    pub fn front(&self) -> &FrontMatter {
        &self.front
    }

    /// One staff per part, in source order.
    pub fn staves(&self) -> &[StaffPlan] {
        &self.staves
    }

    /// Tempo marks, in the order they are reached: the piece's starting
    /// tempo first, then each change.
    pub fn tempos(&self) -> &[PositionedMark<TempoText>] {
        &self.tempos
    }

    /// Form markers, in the order they are reached.
    pub fn sections(&self) -> &[PositionedMark<String>] {
        &self.sections
    }

    /// Chord symbols, in the order they are reached.
    pub fn harmony(&self) -> &[PositionedMark<ChordSymbol>] {
        &self.harmony
    }

    /// Repeats, in the order they are reached.
    pub fn repeats(&self) -> &[RepeatMark] {
        &self.repeats
    }

    /// Open regions, in the order they are reached.
    pub fn open(&self) -> &[OpenMark] {
        &self.open
    }

    /// Every freely-held note, positioned the way a backend anchors a symbol.
    ///
    /// The same fact as [`NotatedItem::free`], said in the other coordinate.
    /// An engraver draws the bracket on the notehead, so it reads the item; a
    /// backend with no bracket prints a word into the measure, so it reads
    /// this. Neither can be derived from the other without walking the score
    /// the other way round.
    pub fn holds(&self) -> &[PositionedMark<musa_score::FreeDuration>] {
        &self.holds
    }
}

/// The title and the four lines around it, as a backend needs them.
///
/// The compiler's `FrontMatter` is the piece's own; this one carries the
/// title too, because a header element wants all five together and a backend
/// that had to reach into two places for one `<titleStmt>` would be reaching
/// into the score for layout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontMatter {
    /// The piece's name, from its `piece` declaration. Always present, though
    /// it may be empty for an unnamed piece.
    pub title: String,
    /// A second line under the title.
    pub subtitle: Option<String>,
    /// Who wrote it.
    pub composer: Option<String>,
    /// Who arranged it.
    pub arranger: Option<String>,
    /// The notice at the foot of the first page.
    pub copyright: Option<String>,
    /// Which performance this file is, when the piece left anything to one.
    ///
    /// A catalogue fact rather than a printed line: a file that leaves a
    /// decision open and does not say which reading it holds cannot be
    /// reproduced, and the formats with somewhere to put a note say so
    /// (`docs/rules/kernel/11-realization.md`, consumer obligation 1).
    pub performance: Option<u64>,
}

/// The plan for one staff (one part).
#[derive(Clone, Debug)]
pub struct StaffPlan {
    pub(super) name: String,
    pub(super) clef: Option<Clef>,
    pub(super) key: Option<KeySignature>,
    pub(super) time_signature: (u32, u32),
    pub(super) measures: Vec<MeasurePlan>,
    pub(super) tempos: Vec<PositionedMark<TempoText>>,
}

impl StaffPlan {
    /// The part name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The written clef, if declared.
    pub fn clef(&self) -> Option<Clef> {
        self.clef
    }

    /// The key signature, if the piece declares a key.
    pub fn key_signature(&self) -> Option<KeySignature> {
        self.key
    }

    /// The time signature as `(numerator, denominator)`.
    pub fn time_signature(&self) -> (u32, u32) {
        self.time_signature
    }

    /// Measures in order, starting at number 1.
    pub fn measures(&self) -> &[MeasurePlan] {
        &self.measures
    }

    /// The tempo markings this staff states *for itself* — polytempo.
    ///
    /// Empty unless the part declares its own tempo, in which case
    /// [`NotationPlan::tempos`] is the conductor's reading and these are what
    /// this staff actually plays. A backend that cannot attach a tempo to a
    /// staff drops them and says so.
    pub fn tempos(&self) -> &[PositionedMark<TempoText>] {
        &self.tempos
    }
}

/// One measure of one staff.
#[derive(Clone, Debug)]
pub struct MeasurePlan {
    pub(super) number: u32,
    pub(super) meter: Meter,
    pub(super) length: MusicalDuration,
    pub(super) time_signature: Option<(u32, u32)>,
    pub(super) key: Option<KeySignature>,
    pub(super) clefs: Vec<ClefChange>,
    pub(super) lanes: Vec<VoiceLane>,
}

impl MeasurePlan {
    /// The 1-based measure number.
    pub fn number(&self) -> u32 {
        self.number
    }

    /// The meter in force across this measure.
    pub fn meter(&self) -> Meter {
        self.meter
    }

    /// How long it is, in whole notes.
    ///
    /// The meter's answer wherever there is a meter, and the passage's own
    /// length where there is not: an unmeasured stretch is one measure that
    /// runs until the next meter or until the music stops, so its duration is
    /// something only the barlines know.
    pub fn duration(&self) -> MusicalDuration {
        self.length
    }

    /// The time signature this measure *prints*, or `None` when it inherits
    /// the one before it.
    ///
    /// A backend writes a time signature exactly where this is `Some`, which
    /// is the same question every backend was asking and none of them could
    /// answer: the first measure prints the piece's meter, and after that a
    /// measure prints one only when it differs from the measure before.
    pub fn time_signature(&self) -> Option<(u32, u32)> {
        self.time_signature
    }

    /// The key signature this measure *prints*, or `None` when it inherits
    /// the one before it. Same question as [`Self::time_signature`], and the
    /// same answer for the same reason.
    pub fn key_signature(&self) -> Option<KeySignature> {
        self.key
    }

    /// Clef changes taking effect in this measure, in onset order.
    pub fn clefs(&self) -> &[ClefChange] {
        &self.clefs
    }

    /// Voice lanes sounding in this measure (one per voice of the part).
    pub fn lanes(&self) -> &[VoiceLane] {
        &self.lanes
    }
}

/// One voice's notated content inside one measure.
#[derive(Clone, Debug)]
pub struct VoiceLane {
    pub(super) voice: VoiceId,
    pub(super) name: String,
    pub(super) items: Vec<NotatedItem>,
    pub(super) slurs: Vec<SlurRange>,
    pub(super) phrases: Vec<PhraseRange>,
    pub(super) hairpins: Vec<HairpinRange>,
    pub(super) points: Vec<PointMark>,
    pub(super) marks: Vec<MarkRange>,
}

impl VoiceLane {
    /// The voice identity in the part.
    pub fn voice(&self) -> VoiceId {
        self.voice
    }

    /// The voice name from the source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Notated items in onset order.
    pub fn items(&self) -> &[NotatedItem] {
        &self.items
    }

    /// Slurs beginning in this measure, in onset order.
    pub fn slurs(&self) -> &[SlurRange] {
        &self.slurs
    }

    /// Phrases beginning in this measure, in onset order.
    pub fn phrases(&self) -> &[PhraseRange] {
        &self.phrases
    }

    /// Hairpins beginning in this measure, in onset order.
    pub fn hairpins(&self) -> &[HairpinRange] {
        &self.hairpins
    }

    /// Notation marks standing at one place in this measure, in onset order.
    pub fn points(&self) -> &[PointMark] {
        &self.points
    }

    /// Notation marks beginning in this measure, in onset order.
    pub fn marks(&self) -> &[MarkRange] {
        &self.marks
    }
}
