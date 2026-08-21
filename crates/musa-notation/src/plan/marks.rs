//! Everything a plan hangs at a position or over a range: repeats, keys,
//! clefs, dynamics, slurs, phrases, hairpins, tuplets.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_score::{Clef, DynamicMark, EventId, Mark, MarkArgument, Metronome, Mode, MusicalDuration};
use num_rational::Ratio;

/// A stretch of the page whose contents or order the performance decides: a
/// mobile's fragments, or an improvised chorus.
///
/// It carries the instruction as text because that is the only thing every
/// backend can print. What is *under* it is the reading this compilation
/// took, which is already in the staves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenMark {
    /// The 1-based measure it opens in.
    pub from: u32,
    /// The 1-based measure it closes in.
    pub to: u32,
    /// The instruction printed over it, as a reader reads it.
    pub text: String,
    /// Which kind of freedom it is, for a backend that can draw one and not
    /// the other, and for the export warnings.
    pub kind: OpenShape,
}

/// The shapes of open region a page can be asked to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenShape {
    /// Fragments in an order the performance chose.
    Mobile,
    /// A frame with unnotated contents.
    Improvise,
    /// A repeat whose count the performance chose: `4–16×` over the repeat
    /// sign.
    ///
    /// It is an open region rather than a field on [`RepeatMark`] because
    /// what a backend does with it is what it does with the other two — print
    /// the instruction as text, because no notation format has a ranged
    /// repeat. One shape, one emitter, and the export warning says the same
    /// thing about all three.
    Passes,
}

/// Repeat barlines and their volta brackets, by measure.
///
/// A repeat crosses the system, so it belongs to the plan rather than to a
/// staff: a backend that let two staves disagree about where `:|` falls would
/// be engraving two scores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatMark {
    /// The measure the `|:` opens.
    pub from: u32,
    /// The measure the `:|` closes, when there are no endings. With endings
    /// each bracket but the last carries the repeat sign instead, which is
    /// where an engraver puts it.
    pub to: u32,
    /// How many times the body is played.
    pub times: u32,
    /// The passes the source asked for, when it left the count open.
    ///
    /// The reason a page can print `4–16×` and play six: [`Self::times`] is
    /// this performance's reading, and this is the composer's instruction.
    pub range: Option<(u32, u32)>,
    /// The volta brackets, in print order. Empty for a plain repeat.
    pub endings: Vec<VoltaMark>,
}

/// One volta bracket: which passes it is labelled with, and which measures it
/// covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoltaMark {
    /// The passes it covers, ascending. `[1]` prints `1.`; `[2, 3]`, `2.–3.`.
    pub passes: Vec<u32>,
    /// The first measure under the bracket.
    pub from: u32,
    /// The last measure under it.
    pub to: u32,
}

impl VoltaMark {
    /// The bracket's label, as the notation formats spell one: `1`, or
    /// `2, 3` for a bracket that covers more than one pass.
    pub fn label(&self) -> String {
        self.passes.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
    }
}

/// A symbol printed at a place in the piece rather than on a note: a form
/// marker, a chord symbol.
///
/// Positions are given as the backends need them — a measure and an offset
/// inside it — because every notation format anchors such a symbol to a bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionedMark<T> {
    /// The 1-based measure it falls in.
    pub measure: u32,
    /// How far into that measure it is, in whole notes.
    pub onset_in_measure: MusicalDuration,
    /// What is printed there.
    pub what: T,
}

impl<T> PositionedMark<T> {
    /// The 1-based beat within the measure, given the meter's beat unit: the
    /// coordinate notation formats anchor a measure-level symbol with.
    ///
    /// The start of a measure is beat 1, and a symbol halfway through a 4/4
    /// bar is beat 3.
    pub fn beat(&self, unit: u32) -> Ratio<i64> {
        self.onset_in_measure.as_ratio() * Ratio::from_integer(i64::from(unit.max(1))) + Ratio::ONE
    }
}

/// A tempo mark as a reader sees it: a metronome mark, a word, or both.
///
/// Both halves are optional because both halves are optional on the page.
/// `Andante` with no number is a tempo marking and prints as one; a bare
/// `1/4 = 92` is one too. Only the metronome half moves a clock, which is
/// why performance reads [`Self::metronome`] and the exporters read whatever
/// they are given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TempoText {
    /// The metronome mark, when the marking gives one.
    pub metronome: Option<Metronome>,
    /// The word printed over the staff, when the marking gives one.
    pub text: Option<String>,
}

/// A key signature element: fifths plus mode. Affects the signature only,
/// never pitch spelling (§6.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySignature {
    /// Sharps (positive) or flats (negative) on the circle of fifths.
    pub fifths: i8,
    /// Major or minor.
    pub mode: Mode,
}

/// A clef change taking effect inside a measure.
///
/// The one context change that is not a barline event, and the reason it is
/// a list on the measure rather than a field on the staff: a cello crossing
/// into treble does it before the note it affects, wherever that note is. It
/// belongs to the staff and not to a voice, because both voices of a staff
/// read the same clef.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClefChange {
    /// Where in the measure it takes effect, in whole notes.
    pub onset_in_measure: MusicalDuration,
    /// The clef the staff reads from there.
    pub clef: Clef,
}

/// A notation mark standing at one place in this lane's measure.
///
/// Positioned by time rather than by event, unlike every other span and mark
/// here, because that is what a point mark is: a breath falls between two
/// notes and belongs to neither.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointMark {
    /// The mark, as written.
    pub mark: Mark,
    /// What was written after its name.
    pub argument: Option<MarkArgument>,
    /// The onset within the measure, in whole notes.
    pub onset_in_measure: MusicalDuration,
}

/// A notation mark covering a run of events, beginning in this lane's
/// measure: a pedal, an ottava.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkRange {
    /// The mark, as written.
    pub mark: Mark,
    /// What was written after its name.
    pub argument: Option<MarkArgument>,
    /// The first event under it.
    pub from: EventId,
    /// The last event under it.
    pub to: EventId,
}

/// A notation mark over a run of items, for the backends that write one
/// inline. Same shape and same reason as [`PhraseMark`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanMark {
    /// The mark, as written.
    pub mark: Mark,
    /// What was written after its name.
    pub argument: Option<MarkArgument>,
    /// The span opens at this item.
    pub start: bool,
    /// The span closes at this item.
    pub stop: bool,
}

/// A hairpin that begins in this lane's measure, by the events it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HairpinRange {
    /// The first event under it.
    pub from: EventId,
    /// The last event under it.
    pub to: EventId,
    /// True for a crescendo, false for a diminuendo.
    pub grows: bool,
    /// The dynamic it arrives at, printed where it closes.
    pub target: DynamicMark,
}

/// A phrase that begins in this lane's measure, by the events it brackets.
///
/// Same reason as `SlurRange`: the items carry start/stop flags for the
/// backends that write a phrase inline, and a backend that anchors it by
/// identity needs both ends and the name at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhraseRange {
    /// The phrase's name, as written.
    pub name: String,
    /// The first event in the phrase.
    pub from: EventId,
    /// The last event in the phrase.
    pub to: EventId,
}

/// A slur that begins in this lane's measure, by the events it joins.
///
/// The items carry the same fact as start/stop flags, which is what an
/// inline syntax like `LilyPond`'s wants; a backend that anchors a slur by
/// identity needs both ends at once, and the far end is often in a later
/// measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlurRange {
    /// The first slurred event.
    pub from: EventId,
    /// The last slurred event.
    pub to: EventId,
}

/// A beam group identifier within a lane's measure (the beat index).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeamGroup(pub u32);

/// Which side of the staff a symbol is printed on.
///
/// The plan decides this so the backends cannot disagree about it: a slur
/// that arched above in MEI and below in `LilyPond` would be two different
/// engravings of one score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Above the staff.
    Above,
    /// Below the staff.
    Below,
}

/// A phrase bracket over a run of items, carrying the name written on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhraseMark {
    /// The phrase's name, as written.
    pub name: String,
    /// The phrase opens at this item.
    pub start: bool,
    /// The phrase closes at this item.
    pub stop: bool,
}

/// A hairpin over a run of items, for the backends that write one inline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HairpinMark {
    /// True for a crescendo, false for a diminuendo.
    pub grows: bool,
    /// The dynamic it arrives at.
    pub target: DynamicMark,
    /// The hairpin opens at this item.
    pub start: bool,
    /// The hairpin closes at this item.
    pub stop: bool,
}

/// A tuplet bracket over a run of items: `num` written values in the time of
/// `den` of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TupletMark {
    /// How many written values the group holds.
    pub num: u32,
    /// How many of those values it lasts.
    pub den: u32,
    /// The bracket opens at this item.
    pub start: bool,
    /// The bracket closes at this item.
    pub stop: bool,
}

/// Where slurs are printed. One rule, because the plan has no stem
/// directions to reason from and a convention beats an inconsistency.
pub const SLUR_PLACEMENT: Placement = Placement::Above;

/// Where articulations are printed.
pub const ARTICULATION_PLACEMENT: Placement = Placement::Above;

/// Where dynamic markings are printed.
pub const DYNAMIC_PLACEMENT: Placement = Placement::Below;
