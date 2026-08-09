//! The `NotationPlan` (roadmap §12.1): measures, voice lanes, beaming, and
//! tie decomposition derived from a `ScoreSnapshot`. Semantic, not
//! typographic: no line breaks, spacing, or engraving decisions.
//!
//! Duration decomposition and measure splitting use exact rational
//! arithmetic; the workspace arithmetic lint is allowed at module scope (see
//! `musa-compiler/src/time.rs` for the totality argument).
#![allow(clippy::arithmetic_side_effects)]

use std::collections::{HashMap, HashSet};

use musa_compiler::{
    BarLines, ChordSymbol, Clef, DynamicMark, EventId, Key, Mark, MarkArgument, Meter, Mode, MusicalDuration,
    MusicalTime, NotatedDuration, Part, Scope, ScoreEvent, ScoreEventKind, ScoreSnapshot, Voice, VoiceId, WrittenPitch,
};
use num_rational::Ratio;

/// Options for notation planning. Empty until a real choice exists (beaming
/// preferences, part extraction); the type fixes the facade shape.
#[derive(Clone, Debug, Default)]
pub struct NotationOptions {}

/// The backend-neutral plan for one score.
#[derive(Clone, Debug)]
pub struct NotationPlan {
    front: FrontMatter,
    staves: Vec<StaffPlan>,
    tempos: Vec<PositionedMark<TempoText>>,
    sections: Vec<PositionedMark<String>>,
    harmony: Vec<PositionedMark<ChordSymbol>>,
    repeats: Vec<RepeatMark>,
    open: Vec<OpenMark>,
    holds: Vec<PositionedMark<musa_compiler::FreeDuration>>,
}

/// A stretch of the page whose contents or order the performance decides: a
/// mobile's fragments, or an improvised chorus.
///
/// It carries the instruction as text because that is the only thing every
/// backend can print. What is *under* it is the reading this compilation
/// took, which is already in the staves — prompt 58's rule, a third time.
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

/// The two shapes of open region a page can be asked to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenShape {
    /// Fragments in an order the performance chose.
    Mobile,
    /// A frame with unnotated contents.
    Improvise,
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
    pub fn holds(&self) -> &[PositionedMark<musa_compiler::FreeDuration>] {
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

/// A tempo mark as a reader sees it: a note value and a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoText {
    /// The beat unit as a fraction of a whole note (`1/4` for a quarter).
    pub beat: Ratio<i64>,
    /// Beats per minute.
    pub bpm: u32,
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

/// The plan for one staff (one part).
#[derive(Clone, Debug)]
pub struct StaffPlan {
    name: String,
    clef: Option<Clef>,
    key: Option<KeySignature>,
    time_signature: (u32, u32),
    measures: Vec<MeasurePlan>,
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

/// One measure of one staff.
#[derive(Clone, Debug)]
pub struct MeasurePlan {
    number: u32,
    meter: Meter,
    time_signature: Option<(u32, u32)>,
    key: Option<KeySignature>,
    clefs: Vec<ClefChange>,
    lanes: Vec<VoiceLane>,
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
    voice: VoiceId,
    name: String,
    items: Vec<NotatedItem>,
    slurs: Vec<SlurRange>,
    phrases: Vec<PhraseRange>,
    hairpins: Vec<HairpinRange>,
    points: Vec<PointMark>,
    marks: Vec<MarkRange>,
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

/// Where one item sits inside a spanning mark: the span may open here, close
/// here, both (a one-item span), or neither.
#[derive(Clone, Copy, Default)]
struct Edges {
    start: bool,
    stop: bool,
}

/// One word per item in the plan snapshots, which are the review surface for
/// this module: two nested `bool` fields per span would bury the change under
/// the shape that carries it.
impl std::fmt::Debug for Edges {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match (self.start, self.stop) {
            (false, false) => "none",
            (true, false) => "start",
            (false, true) => "stop",
            (true, true) => "start+stop",
        })
    }
}

/// One notated symbol: a note, rest, or chord with one spelled duration.
/// Cross-measure events appear once per piece; all pieces of one event share
/// its `EventId` and are chained by tie flags.
#[derive(Clone, Debug)]
pub struct NotatedItem {
    event: EventId,
    kind: NotatedKind,
    onset_in_measure: MusicalDuration,
    duration: NotatedDuration,
    tie: Edges,
    beam: Option<BeamGroup>,
    tuplet: Option<TupletMark>,
    slur: Edges,
    phrase: Option<PhraseMark>,
    hairpin: Option<HairpinMark>,
    dynamic: Option<DynamicMark>,
    articulations: Vec<Mark>,
    graces: Vec<PlannedGrace>,
    spans: Vec<SpanMark>,
    free: Option<musa_compiler::FreeDuration>,
}

/// One grace note leaning on a notated item, ready to print.
///
/// It is *inside* the item rather than beside it because that is what every
/// backend wants: MEI nests a `<graceGrp>` before the note, `MusicXML` writes
/// `<grace/>` notes ahead of it, and `LilyPond` writes `\acciaccatura` as a
/// prefix. A grace has no place of its own in the measure — it has no written
/// duration to occupy one — so a plan that laid graces out among the items
/// would have to invent onsets no backend would use.
///
/// No duration field, and that is the point: how long a grace lasts is the
/// profile's answer (`musa_compiler::GracePolicy`), and the page is silent on
/// it. What the page *does* say — which pitches, in what order, with what
/// marks — is exactly what is here.
#[derive(Clone, Debug)]
pub struct PlannedGrace {
    /// The spelled pitch, verbatim from the score.
    pub pitch: WrittenPitch,
    /// Articulations printed on the grace note itself.
    pub articulations: Vec<Mark>,
}

impl NotatedItem {
    /// The score event this symbol (or tie piece) came from.
    pub fn event(&self) -> EventId {
        self.event
    }

    /// Note, rest, or chord content.
    pub fn kind(&self) -> &NotatedKind {
        &self.kind
    }

    /// The onset within the measure, in whole notes.
    pub fn onset_in_measure(&self) -> MusicalDuration {
        self.onset_in_measure
    }

    /// This piece's spelled duration.
    pub fn duration(&self) -> &NotatedDuration {
        &self.duration
    }

    /// A tie starts here (the event continues in the next piece).
    pub fn tie_start(&self) -> bool {
        self.tie.start
    }

    /// A tie ends here (the event began in an earlier piece).
    pub fn tie_stop(&self) -> bool {
        self.tie.stop
    }

    /// The beam group, when this item beams with its beat neighbors.
    pub fn beam(&self) -> Option<BeamGroup> {
        self.beam
    }

    /// The tuplet bracket this item belongs to, if any.
    pub fn tuplet(&self) -> Option<TupletMark> {
        self.tuplet
    }

    /// A slur begins at this item.
    pub fn slur_start(&self) -> bool {
        self.slur.start
    }

    /// A slur ends at this item.
    pub fn slur_stop(&self) -> bool {
        self.slur.stop
    }

    /// The phrase this item belongs to, if any.
    pub fn phrase(&self) -> Option<&PhraseMark> {
        self.phrase.as_ref()
    }

    /// The hairpin this item is under, if any.
    pub fn hairpin(&self) -> Option<HairpinMark> {
        self.hairpin
    }

    /// The dynamic marking printed at this item, if any.
    pub fn dynamic(&self) -> Option<DynamicMark> {
        self.dynamic
    }

    /// The bounds of a freely-held note: what is drawn, and how far the
    /// bracket reaches. `None` on a note that sounds what it says.
    pub fn free(&self) -> Option<&musa_compiler::FreeDuration> {
        self.free.as_ref()
    }

    /// Articulations printed on this item, in written order.
    pub fn articulations(&self) -> &[Mark] {
        &self.articulations
    }

    /// The grace notes leaning on this item, in written order.
    pub fn graces(&self) -> &[PlannedGrace] {
        &self.graces
    }

    /// The notation marks spanning this item, with the ends they open or
    /// close here.
    pub fn spans(&self) -> &[SpanMark] {
        &self.spans
    }
}

/// Where slurs are printed. One rule, because the plan has no stem
/// directions to reason from and a convention beats an inconsistency.
pub const SLUR_PLACEMENT: Placement = Placement::Above;

/// Where articulations are printed.
pub const ARTICULATION_PLACEMENT: Placement = Placement::Above;

/// Where dynamic markings are printed.
pub const DYNAMIC_PLACEMENT: Placement = Placement::Below;

/// The content of a notated item.
#[derive(Clone, Debug)]
pub enum NotatedKind {
    /// A single written pitch.
    Note {
        /// The spelled pitch, verbatim from the score.
        pitch: WrittenPitch,
    },
    /// A rest.
    Rest,
    /// A chord.
    Chord {
        /// The spelled pitches, in source order.
        pitches: Vec<WrittenPitch>,
    },
}

/// Plan notation for a compiled score.
///
/// # Errors
/// Returns [`NotationError`] when a duration cannot be spelled with standard
/// values and ties inside one measure.
pub fn plan_notation(score: &ScoreSnapshot, _options: &NotationOptions) -> Result<NotationPlan, crate::NotationError> {
    // Notation's own barlines. `Fold` prints a repeat once and plays it
    // twice, so a written measure is not a sounding measure; the snapshot's
    // own `bars()` is the unfolded one, and mixing them would number the page
    // by what it sounds like.
    let fold = Fold::of(score);
    let bars = page_bars(score, &fold);
    let meter = bars.meter_at(MusicalTime::ZERO);
    let key = score.key_at(Scope::Piece, MusicalTime::ZERO).map(key_signature);
    // Every key the piece states, on the page's own clock. A modulation
    // inside a folded repeat prints once, at the measure the page numbers it.
    let keys: Vec<(MusicalTime, KeySignature)> = score
        .keys()
        .changes(Scope::Piece)
        .filter_map(|(at, key)| Some((fold.at(at)?, key_signature(key))))
        .collect();
    let marks = Marks::collect(score);
    let mut staves = Vec::new();
    for (_, part) in score.parts().iter() {
        let clefs: Vec<(MusicalTime, Clef)> = score
            .clefs()
            .changes(Scope::Part { part: part.id().0 })
            .filter_map(|(at, clef)| Some((fold.at(at)?, clef)))
            .collect();
        staves.push(plan_staff(
            score, part, meter, &bars, key, &keys, &clefs, &marks, &fold,
        )?);
    }
    let tempo = score.tempo();
    let mut tempos = vec![positioned(
        &bars,
        MusicalTime::ZERO,
        TempoText {
            beat: tempo.beat,
            bpm: tempo.bpm,
        },
    )];
    tempos.extend(tempo.changes.iter().filter_map(|change| {
        Some(positioned(
            &bars,
            fold.at(change.at)?,
            TempoText {
                beat: change.beat,
                bpm: change.bpm,
            },
        ))
    }));
    let sections = score
        .annotations()
        .sections()
        .iter()
        .filter_map(|section| Some(positioned(&bars, fold.at(section.at)?, section.name.clone())))
        .collect();
    let harmony = score
        .annotations()
        .harmony()
        .iter()
        .filter_map(|chord| Some(positioned(&bars, fold.at(chord.at)?, chord.symbol.clone())))
        .collect();
    let repeats = fold.marks(score, &bars);
    let open = score
        .annotations()
        .open()
        .iter()
        .filter_map(|region| {
            let from = positioned(&bars, fold.at(region.start)?, ()).measure;
            // A region that ends on a barline ends in the measure *before*
            // it: the last measure it covers is the last one with music of
            // its own in it, which is where a reader expects the bracket to
            // close.
            let closes = positioned(&bars, fold.at(region.end)?, ());
            let to = if closes.onset_in_measure == musa_compiler::MusicalDuration::ZERO {
                closes.measure.saturating_sub(1)
            } else {
                closes.measure
            };
            Some(OpenMark {
                from,
                to: to.max(from),
                text: open_text(&region.kind),
                kind: match region.kind {
                    musa_compiler::OpenKind::Mobile { .. } => OpenShape::Mobile,
                    musa_compiler::OpenKind::Improvise { .. } => OpenShape::Improvise,
                },
            })
        })
        .collect();
    let holds = staves
        .iter()
        .flat_map(|staff| staff.measures())
        .flat_map(|measure| {
            measure.lanes().iter().flat_map(move |lane| {
                lane.items().iter().filter_map(move |item| {
                    Some(PositionedMark {
                        measure: measure.number(),
                        onset_in_measure: item.onset_in_measure(),
                        what: *item.free()?,
                    })
                })
            })
        })
        .collect();
    Ok(NotationPlan {
        front: FrontMatter {
            title: score.title().to_string(),
            subtitle: score.front_matter().subtitle.clone(),
            composer: score.front_matter().composer.clone(),
            arranger: score.front_matter().arranger.clone(),
            copyright: score.front_matter().copyright.clone(),
        },
        staves,
        tempos,
        sections,
        harmony,
        repeats,
        open,
        holds,
    })
}

/// What a reader is told, in the words a printed part uses.
fn open_text(kind: &musa_compiler::OpenKind) -> String {
    match kind {
        musa_compiler::OpenKind::Mobile { fragments, order } => {
            let played: Vec<&str> = order
                .iter()
                .filter_map(|index| fragments.get(*index as usize))
                .map(String::as_str)
                .collect();
            format!("any order — this reading: {}", played.join(", "))
        }
        musa_compiler::OpenKind::Improvise { over } => match over {
            Some(changes) => format!("improvise over {changes}"),
            None => "improvise".to_owned(),
        },
    }
}

/// Where the barlines fall **on the page**.
///
/// The snapshot's own [`ScoreSnapshot::bars`] is built over performed time; a
/// repeat prints once and plays twice, so a meter change after a repeat sits
/// at a different measure on the page than in the performance. Folding the
/// changes before folding them into barlines is the whole difference, and it
/// is why the two coordinates are two values rather than one with a flag.
///
/// A change the page does not print — one inside a stretch a repeat swallows
/// — is dropped, as is one that does not land on a page barline. Both are
/// conditions the compiler refuses; rendering has to stay total.
fn page_bars(score: &ScoreSnapshot, fold: &Fold) -> BarLines {
    let mut changes = score.meters().changes(Scope::Piece);
    let opening = changes.next().map_or_else(Meter::default, |(_, meter)| meter);
    let mut bars = BarLines::uniform(opening);
    for (at, meter) in changes {
        if let Some(printed) = fold.at(at) {
            let _ = bars.change(printed, meter);
        }
    }
    bars
}

/// Performed time folded onto the page.
///
/// A repeat plays its body every pass and prints it once, so the page is the
/// timeline with the passes after the first taken out and the rest closed up.
/// Everything below this point — measure numbers, where a mark falls, which
/// measure carries a barline — is in **notated** time; the two coordinates
/// agree exactly where a piece has no repeats, which is why nothing else in
/// this module had to learn the difference.
///
/// This is roadmap §2's layer table doing work: notated position ≠ performed
/// position, and a repeat is the construct that separates them.
#[derive(Debug, Default)]
struct Fold {
    /// Stretches of performed time the page does not print, ascending and
    /// disjoint.
    dropped: Vec<(MusicalTime, MusicalTime)>,
}

impl Fold {
    fn of(score: &ScoreSnapshot) -> Self {
        let mut dropped = Vec::new();
        for repeat in score.annotations().repeats() {
            // Everything from the end of the printed body to the end of the
            // last pass, except the one pass of each ending that prints.
            let mut cursor = repeat.body_end;
            for ending in &repeat.endings {
                if cursor < ending.start {
                    dropped.push((cursor, ending.start));
                }
                cursor = cursor.max(ending.end);
            }
            if cursor < repeat.end {
                dropped.push((cursor, repeat.end));
            }
        }
        dropped.sort_by_key(|interval| interval.0);
        Self { dropped }
    }

    /// Where `at` falls on the page, or `None` when it falls in a stretch the
    /// page does not print.
    fn at(&self, at: MusicalTime) -> Option<MusicalTime> {
        let mut shift = Ratio::ZERO;
        for (from, to) in &self.dropped {
            if at < *from {
                break;
            }
            if at < *to {
                return None;
            }
            shift += to.as_ratio() - from.as_ratio();
        }
        Some(MusicalTime::new(at.as_ratio() - shift))
    }

    /// The same, for the exclusive end of something: a stretch that ends where
    /// a dropped one begins ends *there*, not nowhere.
    fn end_at(&self, at: MusicalTime) -> Option<MusicalTime> {
        let mut shift = Ratio::ZERO;
        for (from, to) in &self.dropped {
            if at <= *from {
                break;
            }
            if at < *to {
                return None;
            }
            shift += to.as_ratio() - from.as_ratio();
        }
        Some(MusicalTime::new(at.as_ratio() - shift))
    }

    /// One voice's events as the page holds them: the passes that print, at
    /// the times they print at.
    fn events(&self, voice: &Voice) -> Vec<ScoreEvent> {
        if self.dropped.is_empty() {
            return voice.events().to_vec();
        }
        voice
            .events()
            .iter()
            .filter_map(|event| {
                let onset = self.at(event.onset)?;
                Some(ScoreEvent { onset, ..event.clone() })
            })
            .collect()
    }

    /// The repeat barlines and volta brackets, in measures.
    fn marks(&self, score: &ScoreSnapshot, bars: &BarLines) -> Vec<RepeatMark> {
        score
            .annotations()
            .repeats()
            .iter()
            .filter_map(|repeat| {
                let from = self.at(repeat.start)?;
                let to = self.end_at(repeat.body_end)?;
                let endings = repeat
                    .endings
                    .iter()
                    .filter_map(|ending| {
                        Some(VoltaMark {
                            passes: ending.passes.clone(),
                            from: bars.at(self.at(ending.start)?).measure,
                            to: bars.closing(self.end_at(ending.end)?),
                        })
                    })
                    .collect();
                Some(RepeatMark {
                    from: bars.at(from).measure,
                    to: bars.closing(to),
                    times: repeat.times,
                    endings,
                })
            })
            .collect()
    }
}

/// The annotation store, indexed the way planning reads it: by the event a
/// symbol belongs to.
#[derive(Default)]
struct Marks {
    tuplets: HashMap<EventId, TupletMark>,
    slur_ends: HashMap<EventId, EventId>,
    slur_stops: HashSet<EventId>,
    dynamics: HashMap<EventId, DynamicMark>,
    articulations: HashMap<EventId, Vec<Mark>>,
    graces: HashMap<EventId, Vec<PlannedGrace>>,
    phrases: HashMap<EventId, PhraseMark>,
    phrase_ends: HashMap<EventId, EventId>,
    hairpins: HashMap<EventId, HairpinMark>,
    hairpin_ends: HashMap<EventId, EventId>,
    spans: HashMap<EventId, Vec<SpanMark>>,
    span_ends: HashMap<EventId, Vec<MarkRange>>,
}

impl Marks {
    fn collect(score: &ScoreSnapshot) -> Self {
        let annotations = score.annotations();
        let mut marks = Self::default();
        for slur in annotations.slurs() {
            marks.slur_ends.insert(slur.from, slur.to);
            marks.slur_stops.insert(slur.to);
        }
        for dynamic in annotations.dynamics() {
            marks.dynamics.insert(dynamic.at, dynamic.mark);
        }
        for articulation in annotations.articulations() {
            marks
                .articulations
                .entry(articulation.at)
                .or_default()
                .push(articulation.mark);
        }
        // Sorted by the payload's own index, not by the order the annotation
        // lane happens to hold: `grace { c5; d5; }` and `grace { d5; c5; }`
        // differ only in that number (docs/kernel/05 N2), so it is what the
        // page has to print by.
        let mut graces: Vec<_> = annotations.graces().iter().collect();
        graces.sort_by_key(|grace| (grace.at.0, grace.index));
        for grace in graces {
            marks.graces.entry(grace.at).or_default().push(PlannedGrace {
                pitch: grace.pitch,
                articulations: grace.articulations.clone(),
            });
        }
        // A group's members are the events between its ends; the snapshot
        // answers which those are, so planning does not re-derive it.
        for phrase in annotations.phrases() {
            marks.phrase_ends.insert(phrase.from, phrase.to);
            for event in score.events_in(phrase.from, phrase.to) {
                marks.phrases.insert(
                    event.id,
                    PhraseMark {
                        name: phrase.name.clone(),
                        start: event.id == phrase.from,
                        stop: event.id == phrase.to,
                    },
                );
            }
        }
        // A note may be under a pedal and an ottava at once, so these are
        // lists where a phrase or a hairpin is one value: nothing stops a
        // player holding the pedal through an octave shift.
        for span in annotations.marks() {
            marks.span_ends.entry(span.from).or_default().push(MarkRange {
                mark: span.mark,
                argument: span.argument.clone(),
                from: span.from,
                to: span.to,
            });
            for event in score.events_in(span.from, span.to) {
                marks.spans.entry(event.id).or_default().push(SpanMark {
                    mark: span.mark,
                    argument: span.argument.clone(),
                    start: event.id == span.from,
                    stop: event.id == span.to,
                });
            }
        }
        for hairpin in annotations.hairpins() {
            marks.hairpin_ends.insert(hairpin.from, hairpin.to);
            for event in score.events_in(hairpin.from, hairpin.to) {
                marks.hairpins.insert(
                    event.id,
                    HairpinMark {
                        grows: hairpin.grows,
                        target: hairpin.target,
                        start: event.id == hairpin.from,
                        stop: event.id == hairpin.to,
                    },
                );
            }
        }
        for tuplet in annotations.tuplets() {
            for event in score.events_in(tuplet.from, tuplet.to) {
                marks.tuplets.insert(
                    event.id,
                    TupletMark {
                        num: tuplet.num,
                        den: tuplet.den,
                        start: event.id == tuplet.from,
                        stop: event.id == tuplet.to,
                    },
                );
            }
        }
        marks
    }

    /// What a tuplet does to a written value: a `3/2` triplet eighth sounds
    /// `1/12` and is printed as the `1/8` it was written as.
    fn symbol_scale(&self, event: EventId) -> Ratio<i64> {
        self.tuplets.get(&event).map_or(Ratio::ONE, |tuplet| {
            Ratio::new(i64::from(tuplet.num), i64::from(tuplet.den))
        })
    }
}

/// Where a positioned symbol falls, in the coordinates the backends use.
fn positioned<T>(bars: &BarLines, at: MusicalTime, what: T) -> PositionedMark<T> {
    let position = bars.at(at);
    PositionedMark {
        measure: position.measure,
        onset_in_measure: position.into,
        what,
    }
}

fn key_signature(key: Key) -> KeySignature {
    KeySignature {
        fifths: key.fifths(),
        mode: key.mode(),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one staff is one plan; splitting the inputs would only move the list"
)]
fn plan_staff(
    score: &ScoreSnapshot,
    part: &Part,
    meter: Meter,
    bars: &BarLines,
    key: Option<KeySignature>,
    keys: &[(MusicalTime, KeySignature)],
    clefs: &[(MusicalTime, Clef)],
    marks: &Marks,
    fold: &Fold,
) -> Result<StaffPlan, crate::NotationError> {
    let lanes: Vec<(VoiceId, String, Vec<ScoreEvent>)> = part
        .voices()
        .map(|(voice_id, voice)| {
            (
                voice_id,
                part.voice_name(voice_id).unwrap_or_default().to_string(),
                fold.events(voice),
            )
        })
        .collect();
    let span = lanes
        .iter()
        .filter_map(|(_, _, events)| events.last())
        .map(|event| (event.onset - MusicalTime::ZERO) + event.notated_duration.value)
        .max()
        .unwrap_or(MusicalDuration::ZERO);
    // Point marks are placed by time, so they go through the fold like clef
    // changes do: a mark inside a repeated passage stands in the measure the
    // page prints, not the one the timeline plays it in (prompt 58).
    let points: Vec<(VoiceId, MusicalTime, &musa_compiler::PointMark)> = score
        .annotations()
        .points()
        .iter()
        .filter(|point| point.part == part.id())
        .filter_map(|point| Some((point.voice, fold.at(point.at)?, point)))
        .collect();
    let mut measures = Vec::new();
    // What the last measure printed, so a measure prints a time signature
    // exactly when it says something the one before it did not.
    let mut printed: Option<Meter> = None;
    let mut printed_key: Option<KeySignature> = None;
    for measure in bars.measures_through(span) {
        let (start, end) = (measure.start, measure.end);
        let mut plans = Vec::with_capacity(lanes.len());
        for (voice_id, name, events) in &lanes {
            let lane = plan_lane(events, measure.meter, measure.length().as_ratio(), start, end, marks)?;
            let here: Vec<PointMark> = points
                .iter()
                .filter(|(voice, at, _)| voice == voice_id && start <= *at && *at < end)
                .map(|(_, at, point)| PointMark {
                    mark: point.mark,
                    argument: point.argument.clone(),
                    onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                })
                .collect();
            plans.push(VoiceLane {
                voice: *voice_id,
                name: name.clone(),
                items: lane.items,
                slurs: lane.slurs,
                phrases: lane.phrases,
                hairpins: lane.hairpins,
                points: here,
                marks: lane.marks,
            });
        }
        let lanes = plans;
        let changed = printed != Some(measure.meter);
        printed = Some(measure.meter);
        // The key in force where this measure opens. Read as "the latest one
        // stated at or before the barline" rather than "one stated exactly
        // here", so a modulation the compiler refused still prints somewhere
        // instead of vanishing: rendering has to stay total.
        let here_key = keys.iter().rfind(|(at, _)| *at <= start).map(|(_, key)| *key);
        let key_changed = here_key.is_some() && here_key != printed_key;
        if here_key.is_some() {
            printed_key = here_key;
        }
        measures.push(MeasurePlan {
            number: measure.number,
            meter: measure.meter,
            time_signature: changed.then(|| (measure.meter.numerator(), measure.meter.denominator())),
            key: key_changed.then_some(here_key).flatten(),
            clefs: clefs
                .iter()
                .skip(1)
                .filter(|(at, _)| *at >= start && *at < end)
                .map(|(at, clef)| ClefChange {
                    onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                    clef: *clef,
                })
                .collect(),
            lanes,
        });
    }
    Ok(StaffPlan {
        name: part.name().to_string(),
        clef: score.clef_at(part.id(), MusicalTime::ZERO),
        key,
        time_signature: (meter.numerator(), meter.denominator()),
        measures,
    })
}

/// The beat unit for beaming: compound meters (`6/8`, `9/8`, `12/8`) beam in
/// groups of three eighths; simple meters beam per notated beat.
fn beam_unit(meter: Meter) -> Ratio<i64> {
    if meter.denominator() == 8 && meter.numerator().is_multiple_of(3) && meter.numerator() > 3 {
        Ratio::new(3, 8)
    } else {
        Ratio::new(1, i64::from(meter.denominator()))
    }
}

/// Notate one voice's events inside one measure.
fn plan_lane(
    events: &[ScoreEvent],
    meter: Meter,
    measure_len: Ratio<i64>,
    start: MusicalTime,
    end: MusicalTime,
    marks: &Marks,
) -> Result<Lane, crate::NotationError> {
    let mut items = Vec::new();
    for event in events {
        let event_end = event.onset + event.notated_duration.value;
        if event_end <= start || event.onset >= end {
            continue;
        }
        let scale = marks.symbol_scale(event.id);
        // The noteheads this event needs here: the pieces the composer wrote,
        // each clipped to this measure and then split into standard values.
        let mut pieces: Vec<(MusicalTime, NotatedDuration)> = Vec::new();
        let mut cursor = event.onset;
        for written in &event.notated_duration.pieces {
            let piece_start = cursor;
            let piece_end = cursor + *written;
            cursor = piece_end;
            if piece_end <= start || piece_start >= end {
                continue;
            }
            let clipped_start = piece_start.max(start);
            let clipped_end = piece_end.min(end);
            let sounding = MusicalDuration::new(clipped_end.as_ratio() - clipped_start.as_ratio());
            let symbol = MusicalDuration::new(sounding.as_ratio() * scale);
            let mut at = clipped_start;
            for value in decompose(event, symbol)? {
                let sounds = MusicalDuration::new(value.value.as_ratio() / scale);
                pieces.push((at, value));
                at = at + sounds;
            }
        }
        let piece_count = pieces.len();
        let continues_after = event_end > end;
        let started_before = event.onset < start;
        let tuplet = marks.tuplets.get(&event.id).copied();
        for (piece_index, (at, piece)) in pieces.into_iter().enumerate() {
            let is_first = piece_index == 0 && !started_before;
            let is_last = piece_index + 1 == piece_count && !continues_after;
            let tied = !matches!(event.kind, ScoreEventKind::Rest);
            let tie = if tied {
                Edges {
                    start: !is_last,
                    stop: !is_first,
                }
            } else {
                Edges::default()
            };
            // A symbol that spells one event only once carries what was
            // written on that event: an accent on the first notehead of a
            // tied pair, not on both.
            items.push(NotatedItem {
                event: event.id,
                kind: kind_of(event),
                onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                duration: piece,
                tie,
                beam: None,
                tuplet: tuplet.map(|tuplet| TupletMark {
                    start: tuplet.start && is_first,
                    stop: tuplet.stop && is_last,
                    ..tuplet
                }),
                slur: Edges {
                    start: is_first && marks.slur_ends.contains_key(&event.id),
                    stop: is_last && marks.slur_stops.contains(&event.id),
                },
                phrase: marks.phrases.get(&event.id).map(|phrase| PhraseMark {
                    start: phrase.start && is_first,
                    stop: phrase.stop && is_last,
                    name: phrase.name.clone(),
                }),
                hairpin: marks.hairpins.get(&event.id).map(|hairpin| HairpinMark {
                    start: hairpin.start && is_first,
                    stop: hairpin.stop && is_last,
                    ..*hairpin
                }),
                dynamic: if is_first {
                    marks.dynamics.get(&event.id).copied()
                } else {
                    None
                },
                articulations: if is_first {
                    marks.articulations.get(&event.id).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                },
                // Graces lean on the *attack*, so they print before the first
                // piece of a tied pair and nowhere else — the same rule as the
                // dynamic, for the same reason.
                graces: if is_first {
                    marks.graces.get(&event.id).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                },
                spans: marks
                    .spans
                    .get(&event.id)
                    .map(|spans| {
                        spans
                            .iter()
                            .map(|span| SpanMark {
                                start: span.start && is_first,
                                stop: span.stop && is_last,
                                mark: span.mark,
                                argument: span.argument.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                // The bracket is drawn from the first notehead of the symbol,
                // like every other thing written once on a tied pair.
                free: if is_first { event.free } else { None },
            });
        }
    }
    assign_beams(&mut items, meter, measure_len);
    let slurs = items
        .iter()
        .filter(|item| item.slur.start)
        .filter_map(|item| {
            marks.slur_ends.get(&item.event).map(|to| SlurRange {
                from: item.event,
                to: *to,
            })
        })
        .collect();
    let phrases = items
        .iter()
        .filter_map(|item| {
            let phrase = item.phrase.as_ref().filter(|phrase| phrase.start)?;
            let to = *marks.phrase_ends.get(&item.event)?;
            Some(PhraseRange {
                name: phrase.name.clone(),
                from: item.event,
                to,
            })
        })
        .collect();
    let hairpins = items
        .iter()
        .filter_map(|item| {
            let hairpin = item.hairpin.filter(|hairpin| hairpin.start)?;
            let to = *marks.hairpin_ends.get(&item.event)?;
            Some(HairpinRange {
                from: item.event,
                to,
                grows: hairpin.grows,
                target: hairpin.target,
            })
        })
        .collect();
    let spans = items
        .iter()
        .flat_map(|item| {
            marks
                .span_ends
                .get(&item.event)
                .into_iter()
                .flatten()
                .filter(|range| item.spans.iter().any(|span| span.start && span.mark == range.mark))
                .cloned()
        })
        .collect();
    Ok(Lane {
        items,
        slurs,
        phrases,
        hairpins,
        marks: spans,
    })
}

/// One lane's plan for one measure, before it is named.
struct Lane {
    items: Vec<NotatedItem>,
    slurs: Vec<SlurRange>,
    phrases: Vec<PhraseRange>,
    hairpins: Vec<HairpinRange>,
    marks: Vec<MarkRange>,
}

fn kind_of(event: &ScoreEvent) -> NotatedKind {
    match &event.kind {
        ScoreEventKind::Note { pitch } => NotatedKind::Note { pitch: *pitch },
        ScoreEventKind::Rest => NotatedKind::Rest,
        ScoreEventKind::Chord { pitches } => NotatedKind::Chord {
            pitches: pitches.clone(),
        },
    }
}

/// Beam consecutive eighth-and-shorter items that stay inside one beat.
fn assign_beams(items: &mut [NotatedItem], meter: Meter, measure_len: Ratio<i64>) {
    let unit = beam_unit(meter);
    let eighth = Ratio::new(1, 8);
    for item in items.iter_mut() {
        // Whether a symbol beams is a question about the symbol: a triplet
        // eighth beams because it is written as an eighth. Where it *ends*
        // is a question about time, and inside a tuplet the two differ.
        let symbol = item.duration.value.as_ratio();
        let short = symbol <= eighth;
        let pitched = !matches!(item.kind, NotatedKind::Rest);
        if !short || !pitched {
            continue;
        }
        let sounding = item.tuplet.map_or(symbol, |tuplet| {
            symbol * Ratio::new(i64::from(tuplet.den), i64::from(tuplet.num))
        });
        let onset = item.onset_in_measure.as_ratio();
        let item_end = onset + sounding;
        if item_end > measure_len {
            continue;
        }
        let beat = onset / unit;
        let beat_index = beat.numer() / beat.denom();
        let beat_end = Ratio::from_integer(beat_index.saturating_add(1)) * unit;
        if item_end <= beat_end {
            item.beam = Some(BeamGroup(u32::try_from(beat_index).unwrap_or(u32::MAX)));
        }
    }
}

/// Decompose a duration inside one measure into standard note values with
/// ties: powers of two (`1/2`, `1/4`, …) and dotted values (`3/4`, `3/8`, …).
/// Anything whose reduced denominator is not a power of two needs tuplets.
fn decompose(event: &ScoreEvent, duration: MusicalDuration) -> Result<Vec<NotatedDuration>, crate::NotationError> {
    let value = duration.as_ratio();
    let denominator = *value.denom();
    if denominator <= 0 || (denominator & (denominator - 1)) != 0 {
        return Err(crate::NotationError::UnspellableDuration {
            event: event.id,
            duration: event.notated_duration.spelling.clone(),
        });
    }
    let mut remaining = value;
    let mut pieces = Vec::new();
    while remaining > Ratio::ZERO {
        let piece = largest_value(remaining);
        pieces.push(NotatedDuration::single(
            MusicalDuration::new(piece),
            event.notated_duration.spelling.clone(),
        ));
        remaining -= piece;
    }
    Ok(pieces)
}

/// The largest standard value (plain or dotted) not exceeding `limit`.
fn largest_value(limit: Ratio<i64>) -> Ratio<i64> {
    // Whole note and smaller: 1, 3/4, 1/2, 3/8, 1/4, 3/16, 1/8, …
    let mut plain = Ratio::from_integer(1);
    while plain > limit {
        plain /= 2;
    }
    let dotted = plain * Ratio::new(3, 2);
    if dotted <= limit { dotted } else { plain }
}
